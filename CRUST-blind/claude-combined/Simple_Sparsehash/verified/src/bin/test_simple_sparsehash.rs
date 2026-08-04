#![allow(unused_imports, non_snake_case)]

use Simple_Sparsehash::simple_sparsehash::{
    sparse_array_free, sparse_array_get, sparse_array_init, sparse_array_set, sparse_dict_free,
    sparse_dict_get, sparse_dict_init, sparse_dict_set, BITCHUNK_SIZE, BITMAP_SIZE, GROUP_SIZE,
    RESIZE_PERCENT, STARTING_SIZE,
};

#[test]
fn test_constants() {
    assert_eq!(GROUP_SIZE, 48);
    assert_eq!(STARTING_SIZE, 32);
    assert_eq!(RESIZE_PERCENT, 80);
    assert_eq!(BITCHUNK_SIZE, 32);
    assert_eq!(BITMAP_SIZE, 2);
}

#[test]
fn test_empty_array_does_not_blow_up() {
    // Mirrors C test_empty_array_does_not_blow_up.
    let arr_box = sparse_array_init(std::mem::size_of::<u64>(), 32);
    assert!(arr_box.is_some());
    let arr = arr_box.unwrap();
    // Get on empty position 0 must return None.
    assert!(sparse_array_get(&arr, 0, None).is_none());
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_cannot_set_outside_bounds() {
    // C: maximum=32, set index 35 must return 0.
    let mut arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    let val: u64 = 666;
    let bytes = val.to_le_bytes();
    assert_eq!(
        sparse_array_set(&mut arr, 35, &bytes, std::mem::size_of::<u64>()),
        0
    );
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_cannot_get_outside_bounds() {
    let arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    assert!(sparse_array_get(&arr, 35, None).is_none());
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_cannot_set_bigger_elements() {
    // elem_size=1 (char), but vlen=8 (sizeof u64) => must reject.
    let mut arr = sparse_array_init(std::mem::size_of::<u8>(), 100).unwrap();
    let val: u64 = 666;
    let bytes = val.to_le_bytes();
    assert_eq!(
        sparse_array_set(&mut arr, 0, &bytes, std::mem::size_of::<u64>()),
        0
    );
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_backwards() {
    let array_size: i32 = 120;
    let mut arr =
        sparse_array_init(std::mem::size_of::<i32>(), array_size as u32).unwrap();
    for i in (0..array_size).rev() {
        let bytes = i.to_le_bytes();
        assert_eq!(
            sparse_array_set(&mut arr, i as u32, &bytes, std::mem::size_of::<i32>()),
            1
        );
        let mut siz = 0usize;
        let returned = sparse_array_get(&arr, i as u32, Some(&mut siz));
        assert!(returned.is_some());
        let r = returned.unwrap();
        assert_eq!(r.len(), 4);
        let v = i32::from_le_bytes(r.try_into().unwrap());
        assert_eq!(v, i);
        assert_eq!(siz, std::mem::size_of::<i32>());
    }
    for i in (0..array_size).rev() {
        let mut siz = 0usize;
        let returned = sparse_array_get(&arr, i as u32, Some(&mut siz));
        let r = returned.unwrap();
        let v = i32::from_le_bytes(r.try_into().unwrap());
        assert_eq!(v, i);
        assert_eq!(siz, std::mem::size_of::<i32>());
    }
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_forwards() {
    let array_size: i32 = 130;
    let mut arr =
        sparse_array_init(std::mem::size_of::<i32>(), array_size as u32).unwrap();
    for i in 0..array_size {
        let bytes = i.to_le_bytes();
        assert_eq!(
            sparse_array_set(&mut arr, i as u32, &bytes, std::mem::size_of::<i32>()),
            1
        );
        let mut siz = 0usize;
        let r = sparse_array_get(&arr, i as u32, Some(&mut siz)).unwrap();
        let v = i32::from_le_bytes(r.try_into().unwrap());
        assert_eq!(v, i);
        assert_eq!(siz, std::mem::size_of::<i32>());
    }
    for i in 0..array_size {
        let mut siz = 0usize;
        let r = sparse_array_get(&arr, i as u32, Some(&mut siz)).unwrap();
        let v = i32::from_le_bytes(r.try_into().unwrap());
        assert_eq!(v, i);
        assert_eq!(siz, std::mem::size_of::<i32>());
    }
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_high_num() {
    // From C test_array_set_high_num: index = GROUP_SIZE - 1 = 47.
    let test_num: i32 = 65555555;
    let index = (GROUP_SIZE - 1) as u32;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 140).unwrap();
    let bytes = test_num.to_le_bytes();
    assert_eq!(
        sparse_array_set(&mut arr, index, &bytes, std::mem::size_of::<i32>()),
        1
    );
    let mut siz = 0usize;
    let returned = sparse_array_get(&arr, index, Some(&mut siz));
    assert!(returned.is_some());
    let r = returned.unwrap();
    let v = i32::from_le_bytes(r.try_into().unwrap());
    assert_eq!(v, 65555555);
    assert_eq!(siz, std::mem::size_of::<i32>());
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_overwrites_old_values() {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 150).unwrap();
    let test_num: i32 = 666;
    let test_num2: i32 = 1024;
    let b1 = test_num.to_le_bytes();
    let b2 = test_num2.to_le_bytes();
    assert_eq!(sparse_array_set(&mut arr, 0, &b1, 4), 1);
    assert_eq!(sparse_array_set(&mut arr, 0, &b2, 4), 1);
    let r = sparse_array_get(&arr, 0, None).unwrap();
    let v = i32::from_le_bytes(r.try_into().unwrap());
    assert_eq!(v, 1024);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_get() {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 200).unwrap();
    let test_num: i32 = 666;
    let mut item_size = 0usize;
    let bytes = test_num.to_le_bytes();
    assert_eq!(sparse_array_set(&mut arr, 0, &bytes, 4), 1);
    let r = sparse_array_get(&arr, 0, Some(&mut item_size)).unwrap();
    let v = i32::from_le_bytes(r.try_into().unwrap());
    assert_eq!(v, 666);
    assert_eq!(item_size, 4);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_get_unset_in_range_returns_none() {
    let arr = sparse_array_init(std::mem::size_of::<i32>(), 100).unwrap();
    // Slot 5 has not been set.
    assert!(sparse_array_get(&arr, 5, None).is_none());
    assert!(sparse_array_get(&arr, 99, None).is_none());
    // index == maximum is allowed (i > maximum is the C check).
    assert!(sparse_array_get(&arr, 100, None).is_none());
    // index > maximum is rejected.
    assert!(sparse_array_get(&arr, 101, None).is_none());
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_at_maximum_index() {
    // C allows setting at index == maximum (because check is `i > maximum`).
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 100).unwrap();
    let bytes = 7i32.to_le_bytes();
    // index = maximum (100) -> allowed by C.
    assert_eq!(sparse_array_set(&mut arr, 100, &bytes, 4), 1);
    let mut siz = 0usize;
    let r = sparse_array_get(&arr, 100, Some(&mut siz)).unwrap();
    let v = i32::from_le_bytes(r.try_into().unwrap());
    assert_eq!(v, 7);
    assert_eq!(siz, 4);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_smaller_value_within_elem_size() {
    // elem_size = 8 (u64), but writing only 2 bytes should work.
    let mut arr = sparse_array_init(8, 32).unwrap();
    let payload: [u8; 2] = [0xAB, 0xCD];
    assert_eq!(sparse_array_set(&mut arr, 3, &payload, 2), 1);
    let mut siz = 0usize;
    let r = sparse_array_get(&arr, 3, Some(&mut siz)).unwrap();
    assert_eq!(siz, 2);
    assert_eq!(r, &[0xAB, 0xCD]);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_dict_set() {
    let mut dict = sparse_dict_init().unwrap();
    let val = b"value";
    assert_eq!(sparse_dict_set(&mut dict, "key", 3, val, val.len()), 1);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_get() {
    let mut dict = sparse_dict_init().unwrap();
    let val = b"value";
    assert_eq!(sparse_dict_set(&mut dict, "key", 3, val, val.len()), 1);
    let mut outsize = 0usize;
    let v = sparse_dict_get(&dict, "key", 3, Some(&mut outsize));
    assert!(v.is_some());
    assert_eq!(outsize, 5);
    assert_eq!(v.unwrap(), b"value");
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_get_missing_returns_none() {
    let dict = sparse_dict_init().unwrap();
    let mut outsize = 999usize;
    let v = sparse_dict_get(&dict, "absent", 6, Some(&mut outsize));
    assert!(v.is_none());
    // outsize untouched on miss
    assert_eq!(outsize, 999);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_overwrite() {
    let mut dict = sparse_dict_init().unwrap();
    let v1 = b"first";
    let v2 = b"second_value";
    assert_eq!(sparse_dict_set(&mut dict, "key", 3, v1, v1.len()), 1);
    assert_eq!(sparse_dict_set(&mut dict, "key", 3, v2, v2.len()), 1);
    let mut outsize = 0usize;
    let v = sparse_dict_get(&dict, "key", 3, Some(&mut outsize)).unwrap();
    assert_eq!(outsize, v2.len());
    assert_eq!(v, b"second_value");
    // bucket_count should not have grown to 2 — overwrite, not insert.
    assert_eq!(dict.bucket_count, 1);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_initial_state() {
    let dict = sparse_dict_init().unwrap();
    assert_eq!(dict.bucket_max, STARTING_SIZE);
    assert_eq!(dict.bucket_count, 0);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_resize_triggers_at_80_percent() {
    // STARTING_SIZE=32, 80% => ratio threshold reached at 26 inserts (26/32 = 0.8125).
    // After resize, bucket_max should double to 64.
    let mut dict = sparse_dict_init().unwrap();
    for i in 0..26 {
        let key = format!("k{}", i);
        let val = format!("v{}", i);
        assert_eq!(
            sparse_dict_set(
                &mut dict,
                &key,
                key.len(),
                val.as_bytes(),
                val.len()
            ),
            1
        );
    }
    assert_eq!(dict.bucket_count, 26);
    assert_eq!(dict.bucket_max, 64);
    // All values still retrievable.
    for i in 0..26 {
        let key = format!("k{}", i);
        let val = format!("v{}", i);
        let mut outsize = 0usize;
        let v = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize)).unwrap();
        assert_eq!(outsize, val.len());
        assert_eq!(v, val.as_bytes());
    }
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_lots_of_set() {
    let mut dict = sparse_dict_init().unwrap();
    let iterations = 1000;
    for i in 0..iterations {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);
        assert_eq!(
            sparse_dict_set(
                &mut dict,
                &key,
                key.len(),
                val.as_bytes(),
                val.len()
            ),
            1
        );
        assert_eq!(dict.bucket_count, (i + 1) as usize);
        let mut outsize = 0usize;
        let v = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize)).unwrap();
        assert_eq!(outsize, val.len());
        assert_eq!(v, val.as_bytes());
    }
    for i in (0..iterations).rev() {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);
        let mut outsize = 0usize;
        let v = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize)).unwrap();
        assert_eq!(outsize, val.len());
        assert_eq!(v, val.as_bytes());
    }
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_distinct_keys_same_prefix() {
    let mut dict = sparse_dict_init().unwrap();
    assert_eq!(sparse_dict_set(&mut dict, "abc", 3, b"one", 3), 1);
    assert_eq!(sparse_dict_set(&mut dict, "abcd", 4, b"two", 3), 1);
    let mut sz = 0usize;
    let v1 = sparse_dict_get(&dict, "abc", 3, Some(&mut sz)).unwrap();
    assert_eq!(sz, 3);
    assert_eq!(v1, b"one");
    let v2 = sparse_dict_get(&dict, "abcd", 4, Some(&mut sz)).unwrap();
    assert_eq!(sz, 3);
    assert_eq!(v2, b"two");
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_value_with_nul_bytes() {
    let mut dict = sparse_dict_init().unwrap();
    let val = b"\x00\x01\x02\x00\x05";
    assert_eq!(sparse_dict_set(&mut dict, "binkey", 6, val, 5), 1);
    let mut sz = 0usize;
    let r = sparse_dict_get(&dict, "binkey", 6, Some(&mut sz)).unwrap();
    assert_eq!(sz, 5);
    assert_eq!(r, val);
    assert_eq!(sparse_dict_free(dict), 1);
}

fn main() {}
