use Simple_Sparsehash::simple_sparsehash::*;

// ---- Sparse Array Tests ----

#[test]
fn test_empty_array_does_not_blow_up() {
    let arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    assert!(sparse_array_get(&arr, 0, None).is_none());
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_cannot_set_outside_bounds() {
    let mut arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    let test_num: u64 = 666;
    let bytes = test_num.to_ne_bytes();
    assert_eq!(sparse_array_set(&mut arr, 35, &bytes, bytes.len()), 0);
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
    let mut arr = sparse_array_init(std::mem::size_of::<u8>(), 100).unwrap();
    let test_num: u64 = 666;
    let bytes = test_num.to_ne_bytes();
    // elem_size is 1 byte, but we're trying to store 8 bytes
    assert_eq!(sparse_array_set(&mut arr, 0, &bytes, bytes.len()), 0);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set() {
    let array_size: u32 = 130;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), array_size).unwrap();

    for i in 0..array_size {
        let val = i as i32;
        let bytes = val.to_ne_bytes();
        assert_eq!(sparse_array_set(&mut arr, i, &bytes, bytes.len()), 1);
        let mut siz: usize = 0;
        let ret = sparse_array_get(&arr, i, Some(&mut siz)).unwrap();
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&ret[..4]);
        assert_eq!(i32::from_ne_bytes(buf), val);
        assert_eq!(siz, std::mem::size_of::<i32>());
    }

    // Verify all values again
    for i in 0..array_size {
        let mut siz: usize = 0;
        let ret = sparse_array_get(&arr, i, Some(&mut siz)).unwrap();
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&ret[..4]);
        assert_eq!(i32::from_ne_bytes(buf), i as i32);
        assert_eq!(siz, std::mem::size_of::<i32>());
    }

    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_backwards() {
    let array_size: u32 = 120;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), array_size).unwrap();

    for i in (0..array_size).rev() {
        let val = i as i32;
        let bytes = val.to_ne_bytes();
        assert_eq!(sparse_array_set(&mut arr, i, &bytes, bytes.len()), 1);
        let mut siz: usize = 0;
        let ret = sparse_array_get(&arr, i, Some(&mut siz)).unwrap();
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&ret[..4]);
        assert_eq!(i32::from_ne_bytes(buf), val);
        assert_eq!(siz, std::mem::size_of::<i32>());
    }

    for i in (0..array_size).rev() {
        let mut siz: usize = 0;
        let ret = sparse_array_get(&arr, i, Some(&mut siz)).unwrap();
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&ret[..4]);
        assert_eq!(i32::from_ne_bytes(buf), i as i32);
        assert_eq!(siz, std::mem::size_of::<i32>());
    }

    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_high_num() {
    let test_num: i32 = 65555555;
    let index = (GROUP_SIZE - 1) as u32;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 140).unwrap();

    let bytes = test_num.to_ne_bytes();
    assert_eq!(sparse_array_set(&mut arr, index, &bytes, bytes.len()), 1);

    let mut siz: usize = 0;
    let ret = sparse_array_get(&arr, index, Some(&mut siz)).unwrap();
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&ret[..4]);
    assert_eq!(i32::from_ne_bytes(buf), test_num);
    assert_eq!(siz, std::mem::size_of::<i32>());

    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_overwrites_old_values() {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 150).unwrap();
    let test_num: i32 = 666;
    let test_num2: i32 = 1024;

    assert_eq!(sparse_array_set(&mut arr, 0, &test_num.to_ne_bytes(), 4), 1);
    assert_eq!(sparse_array_set(&mut arr, 0, &test_num2.to_ne_bytes(), 4), 1);

    let ret = sparse_array_get(&arr, 0, None).unwrap();
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&ret[..4]);
    assert_eq!(i32::from_ne_bytes(buf), 1024);

    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_get() {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 200).unwrap();
    let test_num: i32 = 666;

    assert_eq!(sparse_array_set(&mut arr, 0, &test_num.to_ne_bytes(), 4), 1);

    let mut item_size: usize = 0;
    let ret = sparse_array_get(&arr, 0, Some(&mut item_size)).unwrap();
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&ret[..4]);
    assert_eq!(i32::from_ne_bytes(buf), 666);
    assert_eq!(item_size, std::mem::size_of::<i32>());

    assert_eq!(sparse_array_free(arr), 1);
}

// ---- Sparse Dict Tests ----

#[test]
fn test_dict_init() {
    let dict = sparse_dict_init().unwrap();
    assert_eq!(dict.bucket_max, 32);
    assert_eq!(dict.bucket_count, 0);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_set() {
    let mut dict = sparse_dict_init().unwrap();
    assert_eq!(sparse_dict_set(&mut dict, "key", 3, b"value", 5), 1);
    assert_eq!(dict.bucket_count, 1);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_get() {
    let mut dict = sparse_dict_init().unwrap();
    assert_eq!(sparse_dict_set(&mut dict, "key", 3, b"value", 5), 1);

    let mut outsize: usize = 0;
    let val = sparse_dict_get(&dict, "key", 3, Some(&mut outsize)).unwrap();
    assert_eq!(outsize, 5);
    assert_eq!(&val[..5], b"value");

    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_get_nonexistent() {
    let mut dict = sparse_dict_init().unwrap();
    assert_eq!(sparse_dict_set(&mut dict, "key", 3, b"value", 5), 1);

    let val = sparse_dict_get(&dict, "nonexistent", 11, None);
    assert!(val.is_none());

    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_lots_of_set() {
    let mut dict = sparse_dict_init().unwrap();

    for i in 0..1000 {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);
        assert_eq!(sparse_dict_set(&mut dict, &key, key.len(), val.as_bytes(), val.len()), 1);
        assert_eq!(dict.bucket_count, i + 1);

        let mut outsize: usize = 0;
        let retrieved = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize)).unwrap();
        assert_eq!(outsize, val.len());
        assert_eq!(&retrieved[..outsize], val.as_bytes());
    }

    // Verify all values in reverse
    for i in (0..1000).rev() {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);
        let mut outsize: usize = 0;
        let retrieved = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize)).unwrap();
        assert_eq!(outsize, val.len());
        assert_eq!(&retrieved[..outsize], val.as_bytes());
    }

    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_resize_trigger() {
    // After 25 inserts: bucket_max=32. After 26th: bucket_max=64.
    let mut dict = sparse_dict_init().unwrap();
    for i in 0..25 {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);
        assert_eq!(sparse_dict_set(&mut dict, &key, key.len(), val.as_bytes(), val.len()), 1);
    }
    assert_eq!(dict.bucket_max, 32);
    assert_eq!(dict.bucket_count, 25);

    // 26th insert triggers resize
    let key = "crazy hash25";
    let val = "value25";
    assert_eq!(sparse_dict_set(&mut dict, key, key.len(), val.as_bytes(), val.len()), 1);
    assert_eq!(dict.bucket_max, 64);
    assert_eq!(dict.bucket_count, 26);

    // Verify retrieval still works after resize
    let mut outsize: usize = 0;
    let retrieved = sparse_dict_get(&dict, "crazy hash0", 11, Some(&mut outsize)).unwrap();
    assert_eq!(outsize, 6);
    assert_eq!(&retrieved[..6], b"value0");

    assert_eq!(sparse_dict_free(dict), 1);
}

// ---- Constants Tests ----

#[test]
fn test_constants() {
    assert_eq!(GROUP_SIZE, 48);
    assert_eq!(STARTING_SIZE, 32);
    assert_eq!(RESIZE_PERCENT, 80);
    assert_eq!(BITCHUNK_SIZE, 32);
    assert_eq!(BITMAP_SIZE, 2);
}

fn main() {}
